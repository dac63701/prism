import { useEffect, useMemo, useCallback } from "react";
import { X, Monitor, HardDrive, Film } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/stores/settings";
import { useRecordingStore } from "@/stores/recording";
import RecordingControls from "@/components/common/RecordingControls";
import ScreenPreview from "@/components/common/ScreenPreview";
import SourceSelector from "@/components/common/SourceSelector";

export default function HomePage() {
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const loaded = useSettingsStore((s) => s.loaded);
  const captureTarget = useSettingsStore((s) => s.settings.recording.capture_target);
  const bufferDurationSecs = useSettingsStore((s) => s.settings.recording.buffer_duration_secs);
  const resolution = useSettingsStore((s) => s.settings.recording.resolution);
  const bitrateKbps = useSettingsStore((s) => s.settings.recording.bitrate_kbps);
  const fps = useSettingsStore((s) => s.settings.recording.fps);

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

  function formatElapsed(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

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
    <div className="h-full flex gap-5 px-6 pb-5">
      {/* ── Left: Preview + Controls ── */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Preview */}
        <div className="flex-1 min-h-0 pt-3 pb-4">
          <ScreenPreview recording={isRecording} />
        </div>

        {/* Error banner */}
        {error && (
          <div className="shrink-0 mb-3 flex items-start gap-2 px-4 py-3 rounded-lg bg-red-950/60 border border-red-900/60">
            <p className="text-xs text-red-300 flex-1 leading-relaxed">{error}</p>
            <button
              onClick={clearError}
              className="p-0.5 rounded text-red-400 hover:text-red-200 transition active:scale-90"
            >
              <X className="size-3.5" />
            </button>
          </div>
        )}

        {/* Controls + Info Bar */}
        <div className="flex flex-col items-center gap-4 shrink-0">
          <RecordingControls />

          {/* State text — elapsed time + buffer time */}
          <p className="text-sm text-zinc-500">
            {isRecording
              ? framesReceived === 0
                ? "Recording — waiting for frames..."
                : `${formatElapsed(recordingElapsedSeconds)} · ${formatElapsed(bufferTimeSeconds)} buffered`
              : "Idle"}
          </p>

          {/* Compact info bar */}
          {loaded && (
            <div className="flex items-center gap-4 text-xs text-zinc-500 bg-white/[0.03] border border-white/10 rounded-full px-4 py-1.5">
              {targetLabel && (
                <>
                  <span className="flex items-center gap-1.5">
                    <Monitor className="size-3" />
                    {targetLabel}
                  </span>
                  <span className="text-zinc-700">|</span>
                </>
              )}
              <span className="flex items-center gap-1.5">
                <HardDrive className="size-3" />
                {bufferDurationSecs}s clip
              </span>
              <span className="text-zinc-700">|</span>
              <span className="flex items-center gap-1.5">
                <Film className="size-3" />
                {resolution} · {(bitrateKbps / 1000).toFixed(1).replace(/\.0$/, "")} Mbps · {fps} FPS
              </span>
            </div>
          )}
        </div>
      </div>

      {/* ── Right: Source Selector ── */}
      <div className="w-64 shrink-0 pt-3 pb-5">
        <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-4">
          <SourceSelector
            value={captureTarget}
            onChange={handleSourceChange}
          />
        </div>
      </div>
    </div>
  );
}
