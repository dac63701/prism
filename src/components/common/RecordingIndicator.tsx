import { useRecordingStore } from "@/stores/recording";
import { formatDuration } from "@/stores/clips";

export default function RecordingIndicator() {
  const isRecording = useRecordingStore((s) => s.isRecording);
  const recordingElapsedSeconds = useRecordingStore((s) => s.recordingElapsedSeconds);
  const bufferTimeSeconds = useRecordingStore((s) => s.bufferTimeSeconds);

  if (!isRecording) {
    return (
      <div className="flex items-center gap-2 text-xs text-zinc-600">
        <span className="size-1.5 rounded-full bg-zinc-500" />
        Idle
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 text-xs">
      <span className="size-2 rounded-full bg-red-500 animate-pulse shadow-[0_0_6px_rgba(239,68,68,0.5)]" />
      <span className="font-medium text-white">Recording</span>
      <span className="text-zinc-600">{formatDuration(recordingElapsedSeconds)}</span>
      <span className="text-zinc-600">·</span>
      <span className="text-zinc-600">{formatDuration(bufferTimeSeconds)} buffered</span>
    </div>
  );
}
