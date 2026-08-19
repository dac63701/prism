import { cn } from "@/lib/utils";
import { useRecordingStore } from "@/stores/recording";
import { formatDuration } from "@/stores/clips";

export default function RecordingIndicator({ collapsed = false }: { collapsed?: boolean }) {
  const isRecording = useRecordingStore((s) => s.isRecording);
  const recordingElapsedSeconds = useRecordingStore((s) => s.recordingElapsedSeconds);
  const bufferTimeSeconds = useRecordingStore((s) => s.bufferTimeSeconds);

  if (!isRecording) {
    return (
      <div
        className={cn(
          "flex items-center text-xs text-zinc-600",
          collapsed ? "justify-center" : "gap-2"
        )}
        title="Idle"
      >
        <span className="size-1.5 shrink-0 rounded-full bg-zinc-500" />
        {!collapsed && <span>Idle</span>}
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex items-center text-xs",
        collapsed ? "justify-center" : "gap-2"
      )}
      title={`Recording · ${formatDuration(recordingElapsedSeconds)} · ${formatDuration(bufferTimeSeconds)} buffered`}
    >
      <span className="relative flex size-2 shrink-0">
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-500 opacity-60" />
        <span className="relative inline-flex size-2 rounded-full bg-red-500 shadow-[0_0_6px_rgba(239,68,68,0.5)]" />
      </span>
      {!collapsed && (
        <>
          <span className="truncate font-medium text-white">Recording</span>
          <span className="shrink-0 text-zinc-600 tabular-nums">
            {formatDuration(recordingElapsedSeconds)}
          </span>
        </>
      )}
    </div>
  );
}