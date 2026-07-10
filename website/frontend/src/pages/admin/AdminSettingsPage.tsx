import { useState, useEffect } from "react";
import { Save } from "lucide-react";
import { api } from "@/lib/api";

interface ServerConfig {
  max_upload_size_mb: number;
  default_max_storage_gb: number;
  rate_limit_per_min: number;
  signups_allowed: boolean;
}

export default function AdminSettingsPage() {
  const [config, setConfig] = useState<ServerConfig | null>(null);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  useEffect(() => {
    api.get<ServerConfig>("/api/admin/config")
      .then(setConfig)
      .catch((err) => setMsg({ text: err.message, ok: false }));
  }, []);

  const update = (key: keyof ServerConfig, value: string) => {
    if (!config) return;
    const num = Number(value);
    setConfig({ ...config, [key]: Number.isNaN(num) ? value : num });
    setDirty(true);
  };

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    setMsg(null);
    try {
      await api.put("/api/admin/config", {
        max_upload_size_mb: config.max_upload_size_mb,
        default_max_storage_gb: config.default_max_storage_gb,
        rate_limit_per_min: config.rate_limit_per_min,
        signups_allowed: config.signups_allowed,
      });
      setMsg({ text: "Configuration saved — takes effect immediately", ok: true });
      setDirty(false);
    } catch (err: any) {
      setMsg({ text: err.message, ok: false });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="h-full px-6 py-6 overflow-y-auto">
      <div className="max-w-2xl">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-xl font-semibold text-zinc-100">Server Settings</h1>
            <p className="text-sm text-zinc-500 mt-1">
              Runtime server configuration — changes take effect immediately
            </p>
          </div>
          <button
            onClick={handleSave}
            disabled={!dirty || saving}
            className="flex items-center gap-1.5 bg-zinc-100 text-zinc-950 rounded-lg px-3 py-1.5 text-sm font-medium hover:bg-zinc-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            <Save className="size-3.5" />
            {saving ? "Saving..." : "Save"}
          </button>
        </div>

        {msg && (
          <div className={`mb-4 px-3 py-2 rounded-lg text-sm ${msg.ok ? "bg-emerald-950/50 text-emerald-400 border border-emerald-900/50" : "bg-red-950/50 text-red-400 border border-red-900/50"}`}>
            {msg.text}
          </div>
        )}

        {!config ? (
          <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-6 text-sm text-zinc-500">
            Loading configuration...
          </div>
        ) : (
          <div className="space-y-4">
            <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4 space-y-4">
              <FieldRow label="Max Upload Size" value={String(config.max_upload_size_mb)} unit="MB" onChange={(v) => update("max_upload_size_mb", v)} />
              <FieldRow label="Default Storage Limit" value={String(config.default_max_storage_gb)} unit="GB" onChange={(v) => update("default_max_storage_gb", v)} />
              <FieldRow label="Rate Limit" value={String(config.rate_limit_per_min)} unit="req/min" onChange={(v) => update("rate_limit_per_min", v)} />
              <FieldRow label="Signups Allowed" value={config.signups_allowed ? "true" : "false"} unit="" onChange={(v) => update("signups_allowed", v)} />
            </div>

            <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4 space-y-2">
              <h3 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">Storage Backend</h3>
              <div className="flex justify-between text-sm">
                <span className="text-zinc-500">Type</span>
                <span className="text-zinc-100 font-mono">Local Filesystem</span>
              </div>
            </div>

            {dirty && (
              <p className="text-xs text-amber-400 text-center">
                Unsaved changes
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function FieldRow({ label, value, unit, onChange }: { label: string; value: string; unit: string; onChange: (v: string) => void }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <label className="text-sm text-zinc-400 min-w-[160px]">{label}</label>
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="w-24 bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-sm text-zinc-100 font-mono text-right focus:outline-none focus:ring-1 focus:ring-zinc-600"
        />
        {unit && <span className="text-xs text-zinc-600 w-14">{unit}</span>}
      </div>
    </div>
  );
}
