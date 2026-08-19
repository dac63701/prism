import { useCallback } from "react";
import { Play, Square, Scissors, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { useRecordingStore } from "@/stores/recording";
import { useSettingsStore } from "@/stores/settings";
import { Kbd } from "@/components/ui/kbd";
import { Tooltip } from "@/components/ui/tooltip";

export default function RecordingControls() {
  const isRecording = useRecordingStore((s) => s.isRecording);
  const saving = useRecordingStore((s) => s.saving);
  const starting = useRecordingStore((s) => s.starting);
  const error = useRecordingStore((s) => s.error);
  const startRecording = useRecordingStore((s) => s.startRecording);
  const stopRecording = useRecordingStore((s) => s.stopRecording);
  const saveClip = useRecordingStore((s) => s.saveClip);
  const clearError = useRecordingStore((s) => s.clearError);
  const saveHotkey = useSettingsStore((s) => s.settings.hotkeys.save_clip);

  const handleMainClick = useCallback(() => {
    if (starting) return;
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  }, [starting, isRecording, stopRecording, startRecording]);

  return (
    <div className="flex flex-col items-center gap-3">
      {error && (
        <div className="flex max-w-xs items-center gap-2 rounded-lg border border-red-900/60 bg-red-950/70 px-3 py-2">
          <p className="flex-1 text-[11px] leading-relaxed text-red-300">{error}</p>
          <button
            onClick={clearError}
            className="shrink-0 rounded p-0.5 text-red-400 transition hover:text-red-200 active:scale-90"
            aria-label="Dismiss error"
          >
            <Square className="size-3 rotate-45" />
          </button>
        </div>
      )}

      <div className="flex items-center justify-center gap-5">
        <Tooltip
          label={
            <span className="inline-flex items-center gap-1.5">
              Save clip
              <Kbd>{saveHotkey}</Kbd>
            </span>
          }
          side="top"
        >
          <button
            onClick={() => saveClip()}
            disabled={saving || !isRecording}
            className={cn(
              "size-11 rounded-full flex items-center justify-center transition-all duration-200 active:scale-95",
              "bg-surface border border-border text-zinc-400",
              "hover:bg-white/5 hover:text-zinc-200",
              "disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-surface disabled:hover:text-zinc-400",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
              !isRecording && "pointer-events-none scale-75 opacity-0"
            )}
            title={`Save clip (${saveHotkey})`}
            aria-label="Save clip"
          >
            {saving ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <Scissors className="size-4" />
            )}
          </button>
        </Tooltip>

        <button
          onClick={handleMainClick}
          disabled={starting}
          className={cn(
            "relative size-16 rounded-full flex items-center justify-center transition-all duration-200 active:scale-95",
            "border-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
            starting && "cursor-wait opacity-70",
            isRecording
              ? "bg-red-600 border-red-500 hover:bg-red-500 shadow-[0_0_24px_rgba(239,68,68,0.4)]"
              : "bg-surface border-border hover:bg-white/5"
          )}
          title={
            starting
              ? "Starting..."
              : isRecording
                ? "Stop recording"
                : "Start recording"
          }
          aria-label={
            starting
              ? "Starting..."
              : isRecording
                ? "Stop recording"
                : "Start recording"
          }
        >
          {isRecording && (
            <span className="pointer-events-none absolute inset-0 animate-pulse-ring rounded-full border-2 border-red-500" />
          )}
          {starting ? (
            <Loader2 className="size-6 animate-spin text-zinc-100" />
          ) : isRecording ? (
            <Square className="size-5 fill-current text-zinc-100" />
          ) : (
            <Play className="ml-0.5 size-6 text-zinc-100" />
          )}
        </button>
      </div>
    </div>
  );
}