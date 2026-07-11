import { useState, useEffect, useMemo, useRef } from "react";
import { Monitor, AppWindow, RefreshCw, Check } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "@/lib/utils";

interface DisplayInfo {
  displayId: number;
  width: number;
  height: number;
  isMain: boolean;
}

interface AppInfo {
  pid: number;
  name: string;
  bundleId: string;
  windowCount: number;
}

interface CaptureSources {
  displays: DisplayInfo[];
  applications: AppInfo[];
}

interface SourceSelectorProps {
  value: string;
  onChange: (targetJson: string) => void;
}

export default function SourceSelector({ value, onChange }: SourceSelectorProps) {
  const [sources, setSources] = useState<CaptureSources | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<"display" | "app">("display");
  const mountedRef = useRef(true);

  const loadSources = async () => {
    setLoading(true);
    try {
      const result = await invoke<CaptureSources>("get_capture_sources");
      if (mountedRef.current) {
        setSources(result);
      }
    } catch (err) {
      console.error("Failed to get capture sources:", err);
    }
    if (mountedRef.current) {
      setLoading(false);
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    loadSources();
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Parse current target JSON — memoized to avoid re-parsing on every render
  const currentTarget = useMemo(() => {
    if (!value.trim()) {
      return { kind: "display" as const, id: undefined };
    }
    try {
      const parsed = JSON.parse(value);
      if (typeof parsed === "string" && parsed === "display") {
        return { kind: "display" as const, id: undefined };
      }
      if (typeof parsed === "object" && parsed !== null) {
        if ("display_id" in parsed) {
          return { kind: "display" as const, id: parsed.display_id as number };
        }
        if ("application" in parsed) {
          return { kind: "app" as const, bundleId: parsed.application as string };
        }
      }
      return null;
    } catch {
      return null;
    }
  }, [value]);

  const isDisplaySelected = (displayId: number, isMain: boolean) => {
    if (!currentTarget || currentTarget.kind !== "display") return false;
    if (currentTarget.id === undefined) return isMain;
    return currentTarget.id === displayId;
  };

  const selectDisplay = (displayId: number, isMain: boolean) => {
    if (isMain) {
      onChange('"display"');
    } else {
      onChange(JSON.stringify({ display_id: displayId }));
    }
  };

  const isAppSelected = (bundleId: string) =>
    currentTarget?.kind === "app" && currentTarget.bundleId === bundleId;

  const selectApp = (bundleId: string) => {
    onChange(JSON.stringify({ application: bundleId }));
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium text-zinc-400">Capture Source</h3>
        <button
          onClick={loadSources}
          disabled={loading}
          className="p-1 rounded text-zinc-600 hover:text-zinc-400 hover:bg-zinc-800 transition active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20 disabled:opacity-40"
          title="Refresh sources"
        >
          <RefreshCw className={cn("size-3.5", loading && "animate-spin")} />
        </button>
      </div>

      {/* Tab buttons */}
      <div className="flex gap-1 rounded-lg bg-surface border border-white/10 p-0.5">
        <button
          onClick={() => setActiveTab("display")}
          className={cn(
            "flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md text-xs font-medium transition active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
            activeTab === "display"
              ? "bg-surface-2 text-white"
              : "text-zinc-400 hover:text-white"
          )}
        >
          <Monitor className="size-3.5" />
          Screen
        </button>
        <button
          onClick={() => setActiveTab("app")}
          className={cn(
            "flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md text-xs font-medium transition active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
            activeTab === "app"
              ? "bg-surface-2 text-white"
              : "text-zinc-400 hover:text-white"
          )}
        >
          <AppWindow className="size-3.5" />
          App
        </button>
      </div>

      {/* Source list */}
      <div className="flex flex-col gap-1 max-h-[240px] overflow-y-auto">
        {activeTab === "display" && (
          <>
            {sources?.displays.map((display) => (
              <button
                key={display.displayId}
                onClick={() => selectDisplay(display.displayId, display.isMain)}
                className={cn(
                  "flex items-center gap-2.5 w-full px-3 py-2 rounded-lg text-left transition active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
                  "hover:bg-white/5 border border-transparent",
                  isDisplaySelected(display.displayId, display.isMain) &&
                    "bg-surface-2 border-accent/20"
                )}
              >
                <Monitor className="size-4 shrink-0 text-zinc-500" />
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-medium text-zinc-300 truncate">
                    {display.isMain ? "Main display" : `Display ${display.displayId}`}
                  </p>
                  <p className="text-[11px] text-zinc-600">
                    {display.width}×{display.height}
                  </p>
                </div>
                  {isDisplaySelected(display.displayId, display.isMain) && (
                  <Check className="size-3.5 text-accent shrink-0" />
                )}
              </button>
            ))}
            {(!sources?.displays.length && !loading) && (
              <p className="text-xs text-zinc-600 text-center py-3">
                No displays found
              </p>
            )}
          </>
        )}

        {activeTab === "app" && (
          <>
            {sources?.applications.map((app) => (
              <button
                key={app.bundleId}
                onClick={() => selectApp(app.bundleId)}
                className={cn(
                  "flex items-center gap-2.5 w-full px-3 py-2 rounded-lg text-left transition active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
                  "hover:bg-white/5 border border-transparent",
                  isAppSelected(app.bundleId) &&
                    "bg-surface-2 border-accent/20"
                )}
              >
                <AppWindow className="size-4 shrink-0 text-zinc-500" />
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-medium text-zinc-300 truncate">
                    {app.name}
                  </p>
                  <p className="text-[11px] text-zinc-600 truncate">
                    {app.bundleId}
                    {app.windowCount > 0 && ` · ${app.windowCount}w`}
                  </p>
                </div>
                  {isAppSelected(app.bundleId) && (
                  <Check className="size-3.5 text-accent shrink-0" />
                )}
              </button>
            ))}
            {(!sources?.applications.length && !loading) && (
              <p className="text-xs text-zinc-600 text-center py-3">
                No applications found
              </p>
            )}
          </>
        )}

        {loading && (
          <div className="flex items-center justify-center py-4">
            <RefreshCw className="size-4 text-zinc-500 animate-spin" />
          </div>
        )}
      </div>
    </div>
  );
}
