import { useState, useEffect } from "react";
import { Key, Trash2, Plus, Copy, Check } from "lucide-react";
import { useAuthStore } from "@/stores/auth";
import { api } from "@/lib/api";

interface ApiKeyItem {
  id: string;
  name: string;
  key_prefix: string;
  last_used_at: string | null;
  created_at: string;
}

export default function SettingsPage() {
  const user = useAuthStore((s) => s.user);
  const [keys, setKeys] = useState<ApiKeyItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [newKey, setNewKey] = useState<string | null>(null);
  const [keyName, setKeyName] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [copied, setCopied] = useState(false);
  const [displayName, setDisplayName] = useState("");
  const [passCurrent, setPassCurrent] = useState("");
  const [passNew, setPassNew] = useState("");
  const [passConfirm, setPassConfirm] = useState("");
  const [passMsg, setPassMsg] = useState("");

  useEffect(() => {
    if (user) setDisplayName(user.display_name);
    loadKeys();
  }, [user]);

  const loadKeys = async () => {
    setLoading(true);
    try {
      const data = await api.get<ApiKeyItem[]>("/api/auth/api-keys");
      setKeys(data || []);
    } catch (err) {
      console.error("Failed to load API keys:", err);
    } finally {
      setLoading(false);
    }
  };

  const createKey = async () => {
    try {
      const data = await api.post<{ key: string; key_id: string }>(
        "/api/auth/api-keys",
        { name: keyName }
      );
      setNewKey(data.key);
      setKeyName("");
      loadKeys();
    } catch (err: any) {
      alert(err.message);
    }
  };

  const revokeKey = async (id: string) => {
    if (!confirm("Revoke this API key? This cannot be undone.")) return;
    try {
      await api.delete(`/api/auth/api-keys/${id}`);
      loadKeys();
    } catch (err: any) {
      alert(err.message);
    }
  };

  const handlePasswordChange = async () => {
    if (passNew !== passConfirm) {
      setPassMsg("Passwords do not match");
      return;
    }
    if (passNew.length < 8) {
      setPassMsg("Password must be at least 8 characters");
      return;
    }
    try {
      await api.post("/api/auth/change-password", {
        current_password: passCurrent,
        new_password: passNew,
      });
      setPassMsg("Password changed successfully");
      setPassCurrent("");
      setPassNew("");
      setPassConfirm("");
    } catch (err: any) {
      setPassMsg(err.message);
    }
  };

  const copyKey = () => {
    if (!newKey) return;
    navigator.clipboard.writeText(newKey).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <div className="h-full overflow-y-auto px-6 py-6">
      <div className="max-w-2xl">
        <div className="mb-6">
          <h1 className="text-xl font-semibold text-zinc-100">Settings</h1>
          <p className="text-sm text-zinc-500 mt-1">
            Manage your account and API keys
          </p>
        </div>

        {/* Profile */}
        <section className="mb-8">
          <h2 className="text-sm font-semibold text-zinc-100">Profile</h2>
          <div className="mt-3 border-t border-zinc-800/50 pt-3">
            <div className="flex items-center justify-between py-2">
              <div>
                <p className="text-sm text-zinc-400">Email</p>
                <p className="text-sm text-zinc-100">{user?.email}</p>
              </div>
              <span className="text-xs text-zinc-600 uppercase">{user?.role}</span>
            </div>
          </div>
        </section>

        {/* Password */}
        <section className="mb-8">
          <h2 className="text-sm font-semibold text-zinc-100">Change Password</h2>
          <div className="mt-3 border-t border-zinc-800/50 pt-3 space-y-3 max-w-xs">
            <input
              type="password"
              value={passCurrent}
              onChange={(e) => setPassCurrent(e.target.value)}
              placeholder="Current password"
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
            />
            <input
              type="password"
              value={passNew}
              onChange={(e) => setPassNew(e.target.value)}
              placeholder="New password"
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
            />
            <input
              type="password"
              value={passConfirm}
              onChange={(e) => setPassConfirm(e.target.value)}
              placeholder="Confirm new password"
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-1.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
            />
            <button
              onClick={handlePasswordChange}
              className="bg-zinc-100 text-zinc-950 rounded-lg px-4 py-1.5 text-sm font-medium hover:bg-zinc-200 transition-colors"
            >
              Update Password
            </button>
            {passMsg && (
              <p
                className={`text-xs ${
                  passMsg.includes("successfully") ? "text-green-400" : "text-red-400"
                }`}
              >
                {passMsg}
              </p>
            )}
          </div>
        </section>

        {/* API Keys */}
        <section className="mb-8">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-zinc-100">API Keys</h2>
            <button
              onClick={() => setShowCreate(!showCreate)}
              className="flex items-center gap-1.5 text-xs text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              <Plus className="size-3.5" />
              New Key
            </button>
          </div>
          <div className="mt-3 border-t border-zinc-800/50 pt-3 space-y-2">
            {showCreate && (
              <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-3 space-y-2">
                <input
                  type="text"
                  value={keyName}
                  onChange={(e) => setKeyName(e.target.value)}
                  placeholder="Key name (e.g. Desktop App)"
                  className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
                />
                <button
                  onClick={createKey}
                  className="bg-zinc-100 text-zinc-950 rounded-lg px-3 py-1.5 text-sm font-medium hover:bg-zinc-200"
                >
                  Generate
                </button>

                {newKey && (
                  <div className="mt-2 p-2 rounded bg-zinc-950 border border-emerald-800/50">
                    <p className="text-xs text-emerald-400 mb-1">
                      Copy this key now — you won't see it again!
                    </p>
                    <div className="flex items-center gap-1">
                      <code className="flex-1 text-xs text-zinc-300 font-mono break-all">
                        {newKey}
                      </code>
                      <button
                        onClick={copyKey}
                        className="p-1 text-zinc-500 hover:text-zinc-200"
                      >
                        {copied ? <Check className="size-3.5 text-green-400" /> : <Copy className="size-3.5" />}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            )}

            {loading ? (
              <p className="text-sm text-zinc-600">Loading keys...</p>
            ) : keys.length === 0 ? (
              <p className="text-sm text-zinc-600">No API keys created yet.</p>
            ) : (
              keys.map((key) => (
                <div
                  key={key.id}
                  className="flex items-center justify-between py-2 px-3 rounded-lg bg-zinc-900/50"
                >
                  <div className="flex items-center gap-3">
                    <Key className="size-4 text-zinc-500" />
                    <div>
                      <p className="text-sm text-zinc-200">{key.name || "Unnamed"}</p>
                      <p className="text-[11px] text-zinc-600 font-mono">
                        {key.key_prefix}...
                      </p>
                    </div>
                  </div>
                  <button
                    onClick={() => revokeKey(key.id)}
                    className="p-1.5 rounded text-zinc-600 hover:text-red-400 hover:bg-red-950/50 transition-colors"
                    title="Revoke key"
                  >
                    <Trash2 className="size-3.5" />
                  </button>
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
