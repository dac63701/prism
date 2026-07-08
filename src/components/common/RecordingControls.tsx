import { Play, Square, Scissors, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { useRecordingStore } from "@/stores/recording";

export default function RecordingControls() {
  const isRecording = useRecordingStore((s) => s.isRecording);
  const saving = useRecordingStore((s) => s.saving);
  const starting = useRecordingStore((s) => s.starting);
  const error = useRecordingStore((s) => s.error);
  const startRecording = useRecordingStore((s) => s.startRecording);
  const stopRecording = useRecordingStore((s) => s.stopRecording);
  const saveClip = useRecordingStore((s) => s.saveClip);
  const clearError = useRecordingStore((s) => s.clearError);

  const handleMainClick = () => {
    if (starting) return;
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  };

  return (
    <div className="flex flex-col items-center gap-3">
      {/* Inline error */}
      {error && (
        <div className="flex items-center gap-2 max-w-xs px-3 py-2 rounded-lg bg-red-950/70 border border-red-900/60">
          <p className="text-[11px] text-red-300 leading-relaxed">{error}</p>
          <button
            onClick={clearError}
            className="p-0.5 rounded shrink-0 text-red-400 hover:text-red-200 transition-colors"
          >
            <Square className="size-3 rotate-45" />
          </button>
        </div>
      )}

      <div className="flex items-center justify-center gap-5">
        {/* Clip save button — only visible while recording */}
        <button
          onClick={() => saveClip()}
          disabled={saving || !isRecording}
          className={cn(
            "size-11 rounded-full flex items-center justify-center transition-all duration-200",
            "bg-zinc-800/60 border border-zinc-700/50 text-zinc-400",
            "hover:bg-zinc-700/80 hover:text-zinc-200 hover:border-zinc-600/50",
            "disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-zinc-800/60 disabled:hover:text-zinc-400",
            !isRecording && "opacity-0 pointer-events-none scale-75"
          )}
          title="Save clip"
        >
          <Scissors className="size-4" />
        </button>

        {/* Main record / stop button */}
        <button
          onClick={handleMainClick}
          disabled={starting}
          className={cn(
            "size-16 rounded-full flex items-center justify-center transition-all duration-200",
            "border-2",
            starting && "opacity-70 cursor-wait",
            isRecording
              ? "bg-red-600 border-red-500 hover:bg-red-500 shadow-[0_0_20px_rgba(239,68,68,0.35)]"
              : "bg-zinc-800 border-zinc-700 hover:bg-zinc-700 hover:border-zinc-600"
          )}
          title={
            starting
              ? "Starting..."
              : isRecording
                ? "Stop recording"
                : "Start recording"
          }
        >
          {starting ? (
            <Loader2 className="size-6 text-zinc-100 animate-spin" />
          ) : isRecording ? (
            <Square className="size-5 fill-current text-zinc-100" />
          ) : (
            <Play className="size-6 text-zinc-100 ml-0.5" />
          )}
        </button>
      </div>
    </div>
  );
}
