import { useEffect, useMemo, useCallback } from "react";
import { X, Monitor, HardDrive, Film } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/stores/settings";
import { useRecordingStore } from "@/stores/recording";
import { formatDuration } from "@/stores/clips";
import RecordingControls from "@/components/common/RecordingControls";
import ScreenPreview from "@/components/common/ScreenPreview";
import SourceSelector from "@/components/common/SourceSelector";
import { Badge } from "@/components/ui/brand";
import { useDisplayRefreshRate } from "@/hooks/useDisplayRefreshRate";

export default function HomePage() {
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const loaded = useSettingsStore((s) => s.loaded);
  const captureTarget = useSettingsStore((s) => s.settings.recording.capture_target);
  const bufferDurationSecs = useSettingsStore((s) => s.settings.recording.buffer_duration_secs);
  const resolution = useSettingsStore((s) => s.settings.recording.resolution);
  const bitrateKbps = useSettingsStore((s) => s.settings.recording.bitrate_kbps);
  const fps = useSettingsStore((s) => s.settings.recording.fps);
  const fpsAuto = useSettingsStore((s) => s.settings.recording.fps_auto);
  const displayRefreshRate = useDisplayRefreshRate();
  const effectiveFps = fpsAuto ? displayRefreshRate || fps : fps;

  const isRecording = useRecordingStore((s) => s.isRecording);
  const bufferTimeSeconds = useRecordingStore((s) => s.bufferTimeSeconds);
  const recordingElapsedSeconds = useRecordingStore((s) => s.recordingElapsedSeconds);
  const error = useRecordingStore((s) => s.error);
  const setError = useRecordingStore((s) => s.setError);
  const clearError = useRecordingStore((s) => s.clearError);
  const framesReceived = useRecordingStore((s) => s.framesReceived);

  useEffect(() => {
    if (!loaded) loadSettings();
  }, [loaded, loadSettings]);

  const handleSourceChange = useCallback(async (targetJson: string) => {
    try {
      await invoke("set_capture_target", { targetJson });
      await loadSettings();
    } catch (err) {
      const msg = typeof err === "string" ? err : "Failed to switch capture target";
      setError(msg);
    }
  }, [loadSettings, setError]);

  // Parse current source for display label
  const targetLabel = useMemo(() => {
    if (!captureTarget.trim()) {
      return "Main display";
    }
    try {
      const parsed = JSON.parse(captureTarget);
      if (typeof parsed === "string" && parsed === "display") {
        return "Main display";
      }
      if (typeof parsed === "object" && parsed !== null) {
        if ("display_id" in parsed) {
          return `Display ${parsed.display_id}`;
        }
        if ("application" in parsed) {
          const bundleId = parsed.application as string;
          const parts = bundleId.split(".");
          return parts.length > 2
            ? parts.slice(0, -1).pop() ?? bundleId
            : parts.pop() ?? bundleId;
        }
      }
      return null;
    } catch {
      return null;
    }
  }, [captureTarget]);

  return (
    <div className="flex h-full flex-col gap-5 px-6 pb-5 lg:flex-row">
      {/* ── Left: Preview + Controls ── */}
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="min-h-0 flex-1 pb-4 pt-3">
          <ScreenPreview recording={isRecording} />
        </div>

        {error && (
          <div className="mb-3 flex shrink-0 items-start gap-2 rounded-lg border border-red-900/60 bg-red-950/60 px-4 py-3">
            <p className="flex-1 text-xs leading-relaxed text-red-300">{error}</p>
            <button
              onClick={clearError}
              className="rounded p-0.5 text-red-400 transition hover:text-red-200 active:scale-90"
              aria-label="Dismiss error"
            >
              <X className="size-3.5" />
            </button>
          </div>
        )}

        <div className="flex shrink-0 flex-col items-center gap-4">
          <RecordingControls />

          <p className="text-sm text-zinc-500 tabular-nums">
            {isRecording
              ? framesReceived === 0
                ? "Recording — waiting for frames..."
                : `${formatDuration(recordingElapsedSeconds)} · ${formatDuration(bufferTimeSeconds)} buffered`
              : "Idle"}
          </p>

          {loaded && (
            <div className="flex min-w-0 flex-wrap items-center justify-center gap-2 text-xs text-zinc-500">
              {targetLabel && (
                <Badge className="gap-1.5 px-2.5 py-1 text-[11px]">
                  <Monitor className="size-3" />
                  <span className="truncate">{targetLabel}</span>
                </Badge>
              )}
              <Badge className="gap-1.5 px-2.5 py-1 text-[11px]">
                <HardDrive className="size-3" />
                {bufferDurationSecs}s clip
              </Badge>
              <Badge className="gap-1.5 px-2.5 py-1 text-[11px]">
                <Film className="size-3" />
                {resolution} · {(bitrateKbps / 1000).toFixed(1).replace(/\.0$/, "")} Mbps · {effectiveFps} FPS
              </Badge>
            </div>
          )}
        </div>
      </div>

      {/* ── Right: Source Selector ── */}
      <div className="w-full shrink-0 pb-5 pt-3 lg:w-64">
        <div className="rounded-2xl border border-border bg-white/[0.03] p-4">
          <SourceSelector
            value={captureTarget}
            onChange={handleSourceChange}
          />
        </div>
      </div>
    </div>
  );
}