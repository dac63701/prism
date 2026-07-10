import { useState } from "react";
import { api } from "@/lib/api";

export default function AdminSettingsPage() {
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState("");

  const handleSave = async () => {
    setSaving(true);
    setMsg("");
    try {
      setMsg("Configuration updated");
    } catch (err: any) {
      setMsg(err.message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="max-w-2xl">
        <div className="mb-6">
          <h1 className="text-xl font-semibold text-zinc-100">Server Settings</h1>
          <p className="text-sm text-zinc-500 mt-1">
            Manage server configuration (configured via environment variables)
          </p>
        </div>

        <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4 space-y-3">
          <p className="text-xs text-zinc-500">
            Server configuration is managed through environment variables.
            Changes require a restart to take effect.
          </p>

          {msg && (
            <p
              className={`text-xs ${
                msg.includes("error") ? "text-red-400" : "text-green-400"
              }`}
            >
              {msg}
            </p>
          )}
        </div>

        <div className="mt-4 rounded-xl bg-zinc-900 border border-zinc-800 p-4 space-y-2">
          <h3 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
            Current Configuration
          </h3>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-zinc-500">Max Upload Size</span>
              <span className="text-zinc-100 font-mono">500 MB</span>
            </div>
            <div className="flex justify-between">
              <span className="text-zinc-500">Default Storage Limit</span>
              <span className="text-zinc-100 font-mono">10 GB</span>
            </div>
            <div className="flex justify-between">
              <span className="text-zinc-500">Rate Limit</span>
              <span className="text-zinc-100 font-mono">100 req/min</span>
            </div>
            <div className="flex justify-between">
              <span className="text-zinc-500">Storage Backend</span>
              <span className="text-zinc-100 font-mono">Local Filesystem</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
